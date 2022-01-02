use seed::{prelude::*, *};
use serde_json::json;

use crate::common;

// ------ ------
//     Init
// ------ ------

pub fn init(_: Url, orders: &mut impl Orders<Msg>) -> Model {
    orders.send_msg(Msg::FetchUsers);

    Model {
        users: Vec::new(),
        error_messages: Vec::new(),
    }
}

// ------ ------
//     Model
// ------ ------

pub struct Model {
    users: Users,
    error_messages: Vec<String>,
}

type Users = Vec<User>;

#[derive(serde::Deserialize, Debug)]
pub struct User {
    id: i32,
    name: String,
    sex: i8,
}

// ------ ------
//    Update
// ------ ------

pub enum Msg {
    FetchUsers,
    UsersFetched(Result<Users, String>),
    RequestSession(i32),
    SessionReceived(Result<crate::Session, String>),
}

pub fn update(
    msg: Msg,
    model: &mut Model,
    orders: &mut impl Orders<Msg>,
    session: &mut Option<crate::Session>,
) {
    match msg {
        Msg::FetchUsers => {
            orders.skip().perform_cmd(async {
                match fetch("api/users").await {
                    Ok(response) => {
                        if response.status().is_ok() {
                            match response.json::<Users>().await {
                                Ok(users) => Msg::UsersFetched(Ok(users)),
                                Err(_) => Msg::UsersFetched(Err("deserialization failed".into())),
                            }
                        } else {
                            Msg::UsersFetched(Err("unexpected response".into()))
                        }
                    }
                    Err(_) => Msg::UsersFetched(Err("no connection".into())),
                }
            });
        }
        Msg::UsersFetched(Ok(users)) => {
            model.users = users;
        }
        Msg::UsersFetched(Err(message)) => {
            model
                .error_messages
                .push("Failed to fetch users: ".to_owned() + &message);
        }
        Msg::RequestSession(user_id) => {
            orders.skip().perform_cmd(async move {
                let request = Request::new("api/session")
                    .method(Method::Post)
                    .json(&json!({ "id": user_id }))
                    .expect("serialization failed");
                match fetch(request).await {
                    Ok(response) => {
                        if response.status().is_ok() {
                            match response.json::<crate::Session>().await {
                                Ok(session) => Msg::SessionReceived(Ok(session)),
                                Err(_) => {
                                    Msg::SessionReceived(Err("deserialization failed".into()))
                                }
                            }
                        } else {
                            Msg::SessionReceived(Err("unexpected response".into()))
                        }
                    }
                    Err(_) => Msg::SessionReceived(Err("no connection".into())),
                }
            });
        }
        Msg::SessionReceived(Ok(new_session)) => {
            *session = Some(new_session);
            orders.request_url(crate::Urls::home());
        }
        Msg::SessionReceived(Err(message)) => {
            model
                .error_messages
                .push("Failed to request session: ".to_owned() + &message);
        }
    }
}

// ------ ------
//     View
// ------ ------

pub fn view(model: &Model) -> Node<Msg> {
    div![
        C!["container"],
        C!["has-text-centered"],
        common::view_errors(&model.error_messages),
        &model
            .users
            .iter()
            .map(|user| {
                let user_id = i32::clone(&user.id);
                div![
                    C!["column"],
                    button![
                        C!["button"],
                        C!["is-success"],
                        C!["has-text-weight-bold"],
                        ev(Ev::Click, move |_| Msg::RequestSession(user_id)),
                        &user.name,
                    ]
                ]
            })
            .collect::<Vec<_>>(),
    ]
}
