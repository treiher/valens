use seed::{prelude::*, *};

use crate::data;

// ------ ------
//     Init
// ------ ------

pub fn init(url: Url, orders: &mut impl Orders<Msg>, navbar: &mut crate::Navbar) -> Model {
    let base_url = url.to_hash_base_url();

    orders.notify(data::Msg::FetchUsers);

    navbar.items = vec![("Administration".into(), crate::Urls::new(&base_url).admin())];

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
    div![
        C!["container"],
        C!["has-text-centered"],
        &data_model
            .users
            .iter()
            .map(|user| {
                let user_id = u32::clone(&user.id);
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
