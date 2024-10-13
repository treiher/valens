use seed::{prelude::*, *};

use crate::common;
use crate::data;

// ------ ------
//     Init
// ------ ------

pub fn init(url: Url, orders: &mut impl Orders<Msg>, navbar: &mut crate::Navbar) -> Model {
    orders.notify(data::Msg::ReadUsers);

    navbar.title = String::from("Valens");
    navbar.items = vec![(
        ev(Ev::Click, move |_| {
            crate::Urls::new(url.to_hash_base_url())
                .admin()
                .go_and_load();
        }),
        String::from("gears"),
    )];

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
    LogIn(u32),
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
    if data_model.users.is_empty() && data_model.loading_users {
        common::view_page_loading()
    } else {
        div![
            C!["container"],
            C!["has-text-centered"],
            &data_model
                .users
                .values()
                .map(|user| {
                    let user_id = user.id;
                    div![
                        C!["column"],
                        button![
                            C!["button"],
                            C!["is-link"],
                            ev(Ev::Click, move |_| Msg::LogIn(user_id)),
                            &user.name,
                        ]
                    ]
                })
                .collect::<Vec<_>>(),
        ]
    }
}
