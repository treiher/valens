use seed::{prelude::*, *};
use serde_json::json;

use crate::common;

// ------ ------
//     Init
// ------ ------

pub fn init(url: Url, orders: &mut impl Orders<Msg>, navbar: &mut crate::Navbar) -> Model {
    let base_url = url.to_hash_base_url();

    orders.send_msg(Msg::FetchUsers);

    navbar.items = vec![("Administration".into(), crate::Urls::new(&base_url).admin())];

    Model {
        base_url,
        users: Vec::new(),
        errors: Vec::new(),
    }
}

// ------ ------
//     Model
// ------ ------

pub struct Model {
    base_url: Url,
    users: Users,
    errors: Vec<String>,
}

type Users = Vec<User>;

#[derive(serde::Deserialize, Debug)]
pub struct User {
    id: i32,
    name: String,
    #[allow(dead_code)]
    sex: i8,
}

// ------ ------
//    Update
// ------ ------

pub enum Msg {
    CloseErrorDialog,

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
        Msg::CloseErrorDialog => {
            model.errors.remove(0);
        }

        Msg::FetchUsers => {
            orders
                .skip()
                .perform_cmd(async { common::fetch("api/users", Msg::UsersFetched).await });
        }
        Msg::UsersFetched(Ok(users)) => {
            model.users = users;
        }
        Msg::UsersFetched(Err(message)) => {
            model
                .errors
                .push("Failed to fetch users: ".to_owned() + &message);
        }

        Msg::RequestSession(user_id) => {
            let request = Request::new("api/session")
                .method(Method::Post)
                .json(&json!({ "id": user_id }))
                .expect("serialization failed");
            orders
                .skip()
                .perform_cmd(async move { common::fetch(request, Msg::SessionReceived).await });
        }
        Msg::SessionReceived(Ok(new_session)) => {
            *session = Some(new_session);
            orders.request_url(
                crate::Urls::new(&model.base_url.clone().set_hash_path(&[""; 0])).home(),
            );
        }
        Msg::SessionReceived(Err(message)) => {
            model
                .errors
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
        common::view_error_dialog(&model.errors, &ev(Ev::Click, |_| Msg::CloseErrorDialog)),
        &model
            .users
            .iter()
            .map(|user| {
                let user_id = i32::clone(&user.id);
                div![
                    C!["column"],
                    button![
                        C!["button"],
                        C!["is-link"],
                        ev(Ev::Click, move |_| Msg::RequestSession(user_id)),
                        &user.name,
                    ]
                ]
            })
            .collect::<Vec<_>>(),
    ]
}
