use std::collections::VecDeque;

use log::Level;
use seed::{prelude::*, *};
use valens_domain as domain;
use valens_web_app as web_app;

use crate::{common, data};

// ------ ------
//     Init
// ------ ------

pub fn init(_: Url, orders: &mut impl Orders<Msg>, navbar: &mut crate::Navbar) -> Model {
    orders
        .subscribe(Msg::DataEvent)
        .notify(data::Msg::ReadVersion)
        .notify(data::Msg::ReadUsers);

    navbar.title = String::from("Administration");

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
    AddUser(String, domain::Sex, String),
    EditUser(domain::UserID, String, domain::Sex, String),
    DeleteUser(domain::UserID),
}

// ------ ------
//    Update
// ------ ------

pub enum Msg {
    ShowAddUserDialog,
    ShowEditUserDialog(domain::UserID),
    ShowDeleteUserDialog(domain::UserID),
    CloseUserDialog,

    NameChanged(String),
    SexChanged(String),

    SaveUser,
    DeleteUser(domain::UserID),
    DataEvent(data::Event),

    UpdateApp,
}

const ERROR_NAME_CONFLICT: &str = "A user with this name already exists";

pub fn update(
    msg: Msg,
    model: &mut Model,
    data_model: &data::Model,
    orders: &mut impl Orders<Msg>,
) {
    match msg {
        Msg::ShowAddUserDialog => {
            model.dialog = Dialog::AddUser(String::new(), domain::Sex::FEMALE, String::new());
        }
        Msg::ShowEditUserDialog(id) => {
            let user = &data_model.users[&id];
            let id = user.id;
            let name = user.name.to_string();
            let sex = user.sex;
            model.dialog = Dialog::EditUser(id, name, sex, String::new());
        }
        Msg::ShowDeleteUserDialog(id) => {
            model.dialog = Dialog::DeleteUser(id);
        }
        Msg::CloseUserDialog => {
            model.dialog = Dialog::Hidden;
        }

        Msg::NameChanged(name) => {
            let validated_name = domain::Name::new(&name);
            match model.dialog {
                Dialog::AddUser(ref mut user_name, _, ref mut error) => {
                    match validated_name {
                        Ok(name) => {
                            if data_model.users.values().any(|u| u.name == name) {
                                *error = ERROR_NAME_CONFLICT.into();
                            } else {
                                *error = String::new();
                            }
                        }
                        Err(err) => {
                            *error = err.to_string();
                        }
                    }
                    *user_name = name;
                }
                Dialog::EditUser(ref mut id, ref mut user_name, _, ref mut error) => {
                    match validated_name {
                        Ok(name) => {
                            if data_model
                                .users
                                .values()
                                .any(|u| u.name == name && u.id != *id)
                            {
                                *error = ERROR_NAME_CONFLICT.into();
                            } else {
                                *error = String::new();
                            }
                        }
                        Err(err) => {
                            *error = err.to_string();
                        }
                    }
                    *user_name = name;
                }
                Dialog::Hidden | Dialog::DeleteUser(_) => {
                    panic!();
                }
            }
        }
        Msg::SexChanged(sex) => match model.dialog {
            Dialog::AddUser(_, ref mut user_sex, _)
            | Dialog::EditUser(_, _, ref mut user_sex, _) => {
                *user_sex = sex.parse::<u8>().unwrap().into();
            }
            Dialog::Hidden | Dialog::DeleteUser(_) => {
                panic!();
            }
        },

        Msg::SaveUser => {
            model.loading = true;
            match model.dialog {
                Dialog::AddUser(ref name, sex, _) => {
                    let name = domain::Name::new(name).expect("invalid name");
                    orders.notify(data::Msg::CreateUser(name, sex));
                }
                Dialog::EditUser(id, ref name, sex, _) => {
                    let name = domain::Name::new(name).expect("invalid name");
                    orders.notify(data::Msg::ReplaceUser(domain::User { id, name, sex }));
                }
                Dialog::Hidden | Dialog::DeleteUser(_) => {
                    panic!();
                }
            }
        }
        Msg::DeleteUser(id) => {
            model.loading = true;
            orders.notify(data::Msg::DeleteUser(id));
        }
        Msg::DataEvent(event) => {
            model.loading = false;
            match event {
                data::Event::UserCreatedOk
                | data::Event::UserReplacedOk
                | data::Event::UserDeletedOk => {
                    orders.skip().send_msg(Msg::CloseUserDialog);
                }
                _ => {}
            };
        }

        Msg::UpdateApp => {
            orders.skip().notify(data::Msg::UpdateApp);
        }
    }
}

// ------ ------
//     View
// ------ ------

pub fn view(model: &Model, data_model: &data::Model) -> Node<Msg> {
    div![
        IF![
            matches!(model.dialog, Dialog::EditUser(_, _, _, _) | Dialog::AddUser(_, _, _)) => {
                view_user_dialog(&model.dialog, model.loading)
            }
        ],
        if let Dialog::DeleteUser(id) = model.dialog {
            let user_name = data_model
                .users
                .get(&id)
                .map(|u| u.name.clone().to_string())
                .unwrap_or_default();
            common::view_delete_confirmation_dialog(
                "user",
                &span![&user_name],
                &ev(Ev::Click, move |_| Msg::DeleteUser(id)),
                &ev(Ev::Click, |_| Msg::CloseUserDialog),
                model.loading,
            )
        } else {
            Node::Empty
        },
        view_users(data_model),
        view_versions(data_model),
        view_log(),
        common::view_fab("user-plus", |_| Msg::ShowAddUserDialog)
    ]
}

fn view_users(data_model: &data::Model) -> Vec<Node<Msg>> {
    nodes![
        div![
            C!["container"],
            C!["has-text-centered"],
            common::view_title(&span!["Users"], 3),
        ],
        div![
            C!["table-container"],
            C!["mt-4"],
            table![
                C!["table"],
                C!["is-fullwidth"],
                C!["is-hoverable"],
                thead![tr![th!["Name"], th!["Sex"], th![]]],
                tbody![data_model.users.values().map(|user| {
                    let id = user.id;
                    let sex = &user.sex.to_string();
                    tr![
                        td![&user.name.as_ref()],
                        td![sex],
                        td![
                            a![
                                C!["icon"],
                                C!["mr-2"],
                                ev(Ev::Click, move |_| Msg::ShowEditUserDialog(id)),
                                i![C!["fas fa-user-edit"]]
                            ],
                            a![
                                C!["icon"],
                                C!["ml-2"],
                                ev(Ev::Click, move |_| Msg::ShowDeleteUserDialog(id)),
                                i![C!["fas fa-user-times"]]
                            ]
                        ]
                    ]
                })],
            ]
        ],
    ]
}

fn view_user_dialog(dialog: &Dialog, loading: bool) -> Node<Msg> {
    let title;
    let name;
    let sex;
    let name_error;
    match dialog {
        Dialog::AddUser(user_name, user_sex, error) => {
            title = "Add user";
            name = user_name.clone();
            sex = *user_sex;
            name_error = error;
        }
        Dialog::EditUser(_, user_name, user_sex, error) => {
            title = "Edit user";
            name = user_name.clone();
            sex = *user_sex;
            name_error = error;
        }
        Dialog::Hidden | Dialog::DeleteUser(_) => {
            panic!();
        }
    }
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
                    input![
                        C!["input"],
                        C![IF![!name_error.is_empty() => "is-danger"]],
                        attrs![
                            At::Type => "text",
                            At::Value => name,
                        ]
                    ],
                ],
                p![
                    C!["help"],
                    C!["is-danger"],
                    if name_error.is_empty() {
                        raw!["&nbsp;"]
                    } else {
                        raw![name_error]
                    }
                ]
            ],
            div![
                C!["field"],
                label![C!["label"], "Sex"],
                div![
                    C!["control"],
                    input_ev(Ev::Change, Msg::SexChanged),
                    div![
                        C!["select"],
                        select![
                            option![
                                "female",
                                attrs![
                                    At::Value => 0,
                                    At::Selected => (sex == domain::Sex::FEMALE).as_at_value(),
                                ]
                            ],
                            option![
                                "male",
                                attrs![
                                    At::Value => 1,
                                    At::Selected => (sex == domain::Sex::MALE).as_at_value(),
                                ]
                            ],
                        ],
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
                        ev(Ev::Click, |_| Msg::CloseUserDialog),
                        "Cancel",
                    ]
                ],
                div![
                    C!["control"],
                    button![
                        C!["button"],
                        C!["is-primary"],
                        C![IF![loading => "is-loading"]],
                        attrs![
                            At::Disabled =>
                                (loading || name.is_empty() || !name_error.is_empty())
                                    .as_at_value(),
                        ],
                        ev(Ev::Click, |_| Msg::SaveUser),
                        "Save",
                    ]
                ],
            ],
        ],
        &ev(Ev::Click, |_| Msg::CloseUserDialog),
    )
}

fn view_versions(data_model: &data::Model) -> Node<Msg> {
    div![
        C!["container"],
        C!["mt-6"],
        C!["px-3"],
        common::view_title(&span!["Version"], 3),
        common::view_versions(&data_model.version),
        IF![&data_model.version != env!("VALENS_VERSION") =>
            button![
            C!["button"],
            C!["is-link"],
            C!["mt-5"],
            ev(Ev::Click, |_| Msg::UpdateApp),
            "Update"
            ]
        ],
    ]
}

fn view_log() -> Node<Msg> {
    let entries = match *web_app::log::LOG.lock().unwrap() {
        Some(ref log) => match log.lock().unwrap().read_entries() {
            Ok(entries) => entries,
            Err(err) => VecDeque::from([web_app::log::Entry {
                time: String::new(),
                level: Level::Error,
                message: err.to_string(),
            }]),
        },
        None => VecDeque::from([web_app::log::Entry {
            time: String::new(),
            level: Level::Error,
            message: "Log storage is uninitialized".to_string(),
        }]),
    };
    div![
        C!["container"],
        C!["mt-6"],
        C!["px-3"],
        common::view_title(&span!["Log"], 3),
        if entries.is_empty() {
            common::view_no_data()
        } else {
            div![entries.iter().map(|e| div![
                C!["message"],
                C!["my-1"],
                C![match e.level {
                    Level::Error => "is-danger",
                    Level::Warn => "is-warning",
                    Level::Info => "is-primary",
                    Level::Debug => "is-info",
                    Level::Trace => "is-dark",
                }],
                div![
                    C!["message-body"],
                    C!["p-2"],
                    p![C!["is-size-7"], &e.time],
                    p![&e.message]
                ],
            ])]
        }
    ]
}
