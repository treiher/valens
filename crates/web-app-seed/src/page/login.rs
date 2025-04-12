use seed::{prelude::*, *};

use valens_domain as domain;

use crate::{common, data};

// ------ ------
//     Init
// ------ ------

pub fn init(_url: Url, orders: &mut impl Orders<Msg>, navbar: &mut crate::Navbar) -> Model {
    orders.notify(data::Msg::ReadUsers);

    navbar.title = String::from("Valens");

    Model {}
}

// ------ ------
//     Model
// ------ ------

pub struct Model {}

// ------ ------
//    Update
// ------ ------

pub enum Msg {
    LogIn(domain::UserID),
}

#[allow(clippy::needless_pass_by_value)]
pub fn update(msg: Msg, _model: &mut Model, orders: &mut impl Orders<Msg>) {
    match msg {
        Msg::LogIn(user_id) => {
            orders.skip().notify(data::Msg::RequestSession(user_id));
        }
    }
}

// ------ ------
//     View
// ------ ------

pub fn view(_model: &Model, data_model: &data::Model) -> Node<Msg> {
    if data_model.users.is_empty() && data_model.loading_users > 0 {
        common::view_page_loading()
    } else {
        div![
            C!["container"],
            C!["has-text-centered"],
            if data_model.no_connection {
                nodes![section![
                    C!["hero"],
                    div![C!["hero-body"], common::view_no_connection()]
                ]]
            } else {
                nodes![
                    data_model
                        .users
                        .values()
                        .cloned()
                        .map(|user| {
                            let user_id = user.id;
                            div![
                                C!["column"],
                                button![
                                    C!["button"],
                                    C!["is-link"],
                                    ev(Ev::Click, move |_| Msg::LogIn(user_id)),
                                    &user.name.to_string(),
                                ]
                            ]
                        })
                        .collect::<Vec<_>>()
                ]
            }
        ]
    }
}
