use seed::{prelude::*, *};

use crate::common;

// ------ ------
//     Init
// ------ ------

pub fn init(_: Url, orders: &mut impl Orders<Msg>) -> Model {
    orders.send_msg(Msg::FetchVersion).send_msg(Msg::FetchUsers);

    Model {
        version: String::new(),
        users: Vec::new(),
        dialog: Dialog::Hidden,
        loading: false,
        errors: Vec::new(),
    }
}

// ------ ------
//     Model
// ------ ------

pub struct Model {
    version: String,
    users: Users,
    dialog: Dialog,
    loading: bool,
    errors: Vec<String>,
}

enum Dialog {
    Hidden,
    AddUser(NewUser, String),
    EditUser(User, String),
    DeleteUser(usize),
}

type Users = Vec<User>;

#[derive(serde::Deserialize, Debug, Clone)]
pub struct User {
    id: i32,
    name: String,
    sex: i8,
}

#[derive(serde::Serialize, Debug, Clone)]
pub struct NewUser {
    name: String,
    sex: i8,
}

// ------ ------
//    Update
// ------ ------

pub enum Msg {
    CloseErrorDialog,

    ShowAddUserDialog,
    ShowEditUserDialog(usize),
    ShowDeleteUserDialog(usize),
    CloseUserDialog,

    NameChanged(String),
    SexChanged(String),

    FetchVersion,
    VersionFetched(Result<String, String>),

    FetchUsers,
    UsersFetched(Result<Users, String>),

    DeleteUser(usize),
    UserDeleted(Result<(), String>),

    SaveUser,
    UserSaved(Result<User, String>),
}

const ERROR_EMPTY_NAME: &str = "The name must not be empty";
const ERROR_NAME_CONFLICT: &str = "A user with this name already exists";

pub fn update(msg: Msg, model: &mut Model, orders: &mut impl Orders<Msg>) {
    match msg {
        Msg::CloseErrorDialog => {
            model.errors.remove(0);
        }

        Msg::ShowAddUserDialog => {
            model.dialog = Dialog::AddUser(
                NewUser {
                    name: String::new(),
                    sex: 0,
                },
                String::new(),
            );
        }
        Msg::ShowEditUserDialog(index) => {
            model.dialog = Dialog::EditUser(model.users[index].clone(), String::new());
        }
        Msg::ShowDeleteUserDialog(index) => {
            model.dialog = Dialog::DeleteUser(index);
        }
        Msg::CloseUserDialog => {
            model.dialog = Dialog::Hidden;
        }

        Msg::NameChanged(name) => match model.dialog {
            Dialog::AddUser(ref mut user, ref mut error) => {
                if name.trim().is_empty() {
                    *error = ERROR_EMPTY_NAME.into()
                } else if model.users.iter().any(|u| u.name.trim() == name.trim()) {
                    *error = ERROR_NAME_CONFLICT.into()
                } else {
                    *error = String::new()
                }
                user.name = name;
            }
            Dialog::EditUser(ref mut user, ref mut error) => {
                if name.trim().is_empty() {
                    *error = ERROR_EMPTY_NAME.into()
                } else if model
                    .users
                    .iter()
                    .any(|u| u.name.trim() == name.trim() && u.id != user.id)
                {
                    *error = ERROR_NAME_CONFLICT.into()
                } else {
                    *error = String::new()
                }
                user.name = name;
            }
            Dialog::Hidden | Dialog::DeleteUser(_) => {
                panic!();
            }
        },
        Msg::SexChanged(sex) => match model.dialog {
            Dialog::AddUser(ref mut user, _) => {
                user.sex = sex.parse::<i8>().unwrap();
            }
            Dialog::EditUser(ref mut user, _) => {
                user.sex = sex.parse::<i8>().unwrap();
            }
            Dialog::Hidden | Dialog::DeleteUser(_) => {
                panic!();
            }
        },

        Msg::FetchVersion => {
            orders.perform_cmd(async { common::fetch("api/version", Msg::VersionFetched).await });
        }
        Msg::VersionFetched(Ok(version)) => {
            model.version = version;
        }
        Msg::VersionFetched(Err(message)) => {
            model
                .errors
                .push("Failed to fetch version: ".to_owned() + &message);
        }

        Msg::FetchUsers => {
            orders.perform_cmd(async { common::fetch("api/users", Msg::UsersFetched).await });
        }
        Msg::UsersFetched(Ok(users)) => {
            model.users = users;
        }
        Msg::UsersFetched(Err(message)) => {
            model
                .errors
                .push("Failed to fetch users: ".to_owned() + &message);
        }

        Msg::SaveUser => {
            model.loading = true;
            let request;
            match model.dialog {
                Dialog::AddUser(ref mut user, _) => {
                    user.name = user.name.trim().into();
                    request = Request::new("api/users")
                        .method(Method::Post)
                        .json(user)
                        .expect("serialization failed");
                }
                Dialog::EditUser(ref mut user, _) => {
                    request = Request::new(format!("api/users/{}", user.id))
                        .method(Method::Put)
                        .json(&NewUser {
                            name: user.name.trim().into(),
                            sex: user.sex,
                        })
                        .expect("serialization failed");
                }
                Dialog::Hidden | Dialog::DeleteUser(_) => {
                    panic!();
                }
            }
            orders.perform_cmd(async move { common::fetch(request, Msg::UserSaved).await });
        }
        Msg::UserSaved(Ok(_)) => {
            model.loading = false;
            orders
                .skip()
                .send_msg(Msg::FetchUsers)
                .send_msg(Msg::CloseUserDialog);
        }
        Msg::UserSaved(Err(message)) => {
            model.loading = false;
            model
                .errors
                .push("Failed to save user: ".to_owned() + &message);
        }

        Msg::DeleteUser(index) => {
            model.loading = true;
            let id = model.users[index].id;
            let request = Request::new(format!("api/users/{}", id)).method(Method::Delete);
            orders.perform_cmd(
                async move { common::fetch_no_content(request, Msg::UserDeleted).await },
            );
        }
        Msg::UserDeleted(Ok(_)) => {
            model.loading = false;
            orders
                .skip()
                .send_msg(Msg::FetchUsers)
                .send_msg(Msg::CloseUserDialog);
        }
        Msg::UserDeleted(Err(message)) => {
            model.loading = false;
            model
                .errors
                .push("Failed to delete user: ".to_owned() + &message);
        }
    }
}

// ------ ------
//     View
// ------ ------

pub fn view(model: &Model) -> Node<Msg> {
    div![
        IF![
            matches!(model.dialog, Dialog::EditUser(_, _) | Dialog::AddUser(_, _)) => {
                view_user_dialog(&model.dialog, model.loading)
            }
        ],
        if let Dialog::DeleteUser(index) = model.dialog {
            common::view_delete_confirmation_dialog(
                "user",
                &ev(Ev::Click, move |_| Msg::DeleteUser(index)),
                &ev(Ev::Click, |_| Msg::CloseUserDialog),
                model.loading,
            )
        } else {
            Node::Empty
        },
        common::view_error_dialog(&model.errors, &ev(Ev::Click, |_| Msg::CloseErrorDialog)),
        div![
            C!["table-container"],
            C!["mt-4"],
            table![
                C!["table"],
                C!["is-fullwidth"],
                C!["is-hoverable"],
                C!["has-text-centered"],
                thead![tr![th!["Name"], th!["Sex"], th![]]],
                tbody![&model
                    .users
                    .iter()
                    .enumerate()
                    .map(|(i, user)| {
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
                                    ev(Ev::Click, move |_| Msg::ShowEditUserDialog(i)),
                                    i![C!["fas fa-user-edit"]]
                                ],
                                a![
                                    C!["icon"],
                                    C!["ml-2"],
                                    ev(Ev::Click, move |_| Msg::ShowDeleteUserDialog(i)),
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
        div![
            C!["navbar"],
            C!["is-fixed-bottom"],
            div![
                C!["container"],
                C!["is-flex-direction-column"],
                div![
                    C!["columns"],
                    C!["is-mobile"],
                    div![
                        C!["column"],
                        C!["has-text-centered"],
                        C!["has-text-grey"],
                        raw![&("<b>Valens</b> ".to_owned() + &model.version)],
                    ],
                ]
            ],
        ],
    ]
}

fn view_user_dialog(dialog: &Dialog, loading: bool) -> Node<Msg> {
    let title;
    let name;
    let sex;
    let name_error;
    match dialog {
        Dialog::AddUser(ref user, ref error) => {
            title = "Add user";
            name = &user.name;
            sex = user.sex;
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
        "success",
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
                    if !name_error.is_empty() {
                        raw![name_error]
                    } else {
                        raw!["&nbsp;"]
                    }
                ]
            ],
            div![
                C!["field"],
                label![C!["label"], "Sex"],
                div![
                    C!["control"],
                    input_ev(Ev::Input, Msg::SexChanged),
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
                        ev(Ev::Click, |_| Msg::CloseUserDialog),
                        "Cancel",
                    ]
                ],
                div![
                    C!["control"],
                    button![
                        C!["button"],
                        C!["is-success"],
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
