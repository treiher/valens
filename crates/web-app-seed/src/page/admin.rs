use seed::{prelude::*, *};
use valens_domain as domain;

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
    AddUser(String, u8, String),
    EditUser(domain::User, String),
    DeleteUser(u32),
}

// ------ ------
//    Update
// ------ ------

pub enum Msg {
    ShowAddUserDialog,
    ShowEditUserDialog(u32),
    ShowDeleteUserDialog(u32),
    CloseUserDialog,

    NameChanged(String),
    SexChanged(String),

    SaveUser,
    DeleteUser(u32),
    DataEvent(data::Event),

    UpdateApp,
}

const ERROR_EMPTY_NAME: &str = "The name must not be empty";
const ERROR_NAME_CONFLICT: &str = "A user with this name already exists";

pub fn update(
    msg: Msg,
    model: &mut Model,
    data_model: &data::Model,
    orders: &mut impl Orders<Msg>,
) {
    match msg {
        Msg::ShowAddUserDialog => {
            model.dialog = Dialog::AddUser(String::new(), 0, String::new());
        }
        Msg::ShowEditUserDialog(id) => {
            model.dialog = Dialog::EditUser(data_model.users[&id].clone(), String::new());
        }
        Msg::ShowDeleteUserDialog(id) => {
            model.dialog = Dialog::DeleteUser(id);
        }
        Msg::CloseUserDialog => {
            model.dialog = Dialog::Hidden;
        }

        Msg::NameChanged(name) => match model.dialog {
            Dialog::AddUser(ref mut user_name, _, ref mut error) => {
                if name.trim().is_empty() {
                    *error = ERROR_EMPTY_NAME.into();
                } else if data_model
                    .users
                    .values()
                    .any(|u| u.name.trim() == name.trim())
                {
                    *error = ERROR_NAME_CONFLICT.into();
                } else {
                    *error = String::new();
                }
                *user_name = name;
            }
            Dialog::EditUser(ref mut user, ref mut error) => {
                if name.trim().is_empty() {
                    *error = ERROR_EMPTY_NAME.into();
                } else if data_model
                    .users
                    .values()
                    .any(|u| u.name.trim() == name.trim() && u.id != user.id)
                {
                    *error = ERROR_NAME_CONFLICT.into();
                } else {
                    *error = String::new();
                }
                user.name = name;
            }
            Dialog::Hidden | Dialog::DeleteUser(_) => {
                panic!();
            }
        },
        Msg::SexChanged(sex) => match model.dialog {
            Dialog::AddUser(_, ref mut user_sex, _) => {
                *user_sex = sex.parse::<u8>().unwrap();
            }
            Dialog::EditUser(ref mut user, _) => {
                user.sex = sex.parse::<u8>().unwrap();
            }
            Dialog::Hidden | Dialog::DeleteUser(_) => {
                panic!();
            }
        },

        Msg::SaveUser => {
            model.loading = true;
            match model.dialog {
                Dialog::AddUser(ref mut user_name, ref mut user_sex, _) => {
                    *user_name = user_name.trim().into();
                    orders.notify(data::Msg::CreateUser(user_name.clone(), *user_sex));
                }
                Dialog::EditUser(ref mut user, _) => {
                    user.name = user.name.trim().into();
                    orders.notify(data::Msg::ReplaceUser(user.clone()));
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
            matches!(model.dialog, Dialog::EditUser(_, _) | Dialog::AddUser(_, _, _)) => {
                view_user_dialog(&model.dialog, model.loading)
            }
        ],
        if let Dialog::DeleteUser(id) = model.dialog {
            common::view_delete_confirmation_dialog(
                "user",
                &ev(Ev::Click, move |_| Msg::DeleteUser(id)),
                &ev(Ev::Click, |_| Msg::CloseUserDialog),
                model.loading,
            )
        } else {
            Node::Empty
        },
        view_users(data_model),
        view_versions(data_model)
    ]
}

fn view_users(data_model: &data::Model) -> Vec<Node<Msg>> {
    nodes![
        div![
            C!["container"],
            C!["has-text-centered"],
            C!["mx-3"],
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
                tbody![&data_model
                    .users
                    .values()
                    .map(|user| {
                        let id = user.id;
                        let sex = &user.sex.to_string();
                        let sex = match &user.sex {
                            0 => "female",
                            1 => "male",
                            _ => sex,
                        };
                        tr![
                            td![&user.name],
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
                    })
                    .collect::<Vec<_>>(),],
            ]
        ],
        button![
            C!["button"],
            C!["is-fab-navbar"],
            C!["is-medium"],
            C!["is-link"],
            ev(Ev::Click, |_| Msg::ShowAddUserDialog),
            span![C!["icon"], i![C!["fas fa-user-plus"]]]
        ],
    ]
}

fn view_user_dialog(dialog: &Dialog, loading: bool) -> Node<Msg> {
    let title;
    let name;
    let sex;
    let name_error;
    match dialog {
        Dialog::AddUser(ref user_name, ref user_sex, ref error) => {
            title = "Add user";
            name = user_name;
            sex = *user_sex;
            name_error = error;
        }
        Dialog::EditUser(ref user, ref error) => {
            title = "Edit user";
            name = &user.name;
            sex = user.sex;
            name_error = error;
        }
        Dialog::Hidden | Dialog::DeleteUser(_) => {
            panic!();
        }
    }
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
                                    At::Selected => (sex == 0).as_at_value(),
                                ]
                            ],
                            option![
                                "male",
                                attrs![
                                    At::Value => 1,
                                    At::Selected => (sex == 1).as_at_value(),
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
        C!["mx-3"],
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
